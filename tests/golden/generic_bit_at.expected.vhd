library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_generic_bit_at is
  generic (
    gl_g0 : positive := 8
  );
  port (
    gl_p0_data : in unsigned(gl_g0 - 1 downto 0);
    gl_p1_high : out std_logic
  );
end entity gl_m0_generic_bit_at;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_generic_bit_at is
  signal gl_s1_high : std_logic;
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  function gl_bit_at(value : unsigned; index : natural) return std_logic is
  begin
    return value(index);
  end function gl_bit_at;
begin
  gl_comb_0 : process(all)
  begin
    gl_s1_high <= gl_bit_at(gl_p0_data, (gl_g0 - 1));
  end process gl_comb_0;
  gl_p1_high <= gl_s1_high;
end architecture rtl;
