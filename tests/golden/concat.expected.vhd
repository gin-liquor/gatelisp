library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_concat_example is
  port (
    gl_p0_high : in unsigned(7 downto 0);
    gl_p1_low : in unsigned(7 downto 0);
    gl_p2_value : out unsigned(15 downto 0)
  );
end entity gl_m0_concat_example;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_concat_example is
  signal gl_s2_value : unsigned(15 downto 0);
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
