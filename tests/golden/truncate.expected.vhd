library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_truncate_example is
  port (
    gl_p0_input : in unsigned(15 downto 0);
    gl_p1_value : out unsigned(7 downto 0)
  );
end entity gl_m0_truncate_example;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_truncate_example is
  signal gl_s1_value : unsigned(7 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  function gl_truncate_unsigned(value : unsigned; size : positive) return unsigned is
  begin
    return value(value'low + size - 1 downto value'low);
  end function gl_truncate_unsigned;
begin
  gl_comb_0 : process(all)
  begin
    gl_s1_value <= gl_truncate_unsigned(gl_p0_input, 8);
  end process gl_comb_0;
  gl_p1_value <= gl_s1_value;
end architecture rtl;
