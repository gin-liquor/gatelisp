library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_generic_concat is
  generic (
    gl_g0 : positive := 8
  );
  port (
    gl_p0_high : in unsigned(gl_g0 - 1 downto 0);
    gl_p1_low : in unsigned(gl_g0 - 1 downto 0);
    gl_p2_value : out unsigned((gl_g0 + gl_g0) - 1 downto 0)
  );
end entity gl_m0_generic_concat;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_generic_concat is
  signal gl_s2_value : unsigned((gl_g0 + gl_g0) - 1 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s2_value <= unsigned((std_logic_vector(gl_p0_high) & std_logic_vector(gl_p1_low)));
  end process gl_comb_0;
  gl_p2_value <= gl_s2_value;
end architecture rtl;
